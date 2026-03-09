#[derive(Clone)]
struct AiThreadGitActionContext {
    repo_root: std::path::PathBuf,
    thread_id: String,
    branch_name: String,
    start_mode: AiNewThreadStartMode,
}

impl DiffViewer {
    fn ai_current_thread_git_action_context(
        &self,
        action_description: &str,
    ) -> Result<AiThreadGitActionContext, String> {
        if self.git_controls_busy() {
            return Err("Another workspace action is in progress.".to_string());
        }

        let Some(thread_id) = self.current_ai_thread_id() else {
            return Err(format!("Select a thread before {action_description}."));
        };
        let Some(repo_root) = self.ai_workspace_cwd() else {
            return Err(format!("Open a workspace before {action_description}."));
        };
        let Some(start_mode) = self.ai_thread_start_mode(thread_id.as_str()) else {
            return Err(format!(
                "Unable to resolve the selected thread before {action_description}."
            ));
        };

        let branch_name = self
            .workspace_targets
            .iter()
            .find(|target| target.root == repo_root)
            .map(|target| target.branch_name.clone())
            .unwrap_or_else(|| {
                self.primary_checked_out_branch_name()
                    .unwrap_or(self.branch_name.as_str())
                    .to_string()
            });
        let normalized_branch = branch_name.trim();
        if normalized_branch.is_empty()
            || matches!(normalized_branch, "detached" | "unknown")
        {
            return Err(format!("Activate a branch before {action_description}."));
        }

        Ok(AiThreadGitActionContext {
            repo_root,
            thread_id,
            branch_name,
            start_mode,
        })
    }

    pub(super) fn ai_publish_blocker(&self) -> Option<String> {
        let context = match self.ai_current_thread_git_action_context("publishing") {
            Ok(context) => context,
            Err(reason) => return Some(reason),
        };
        if context.start_mode != AiNewThreadStartMode::Local {
            return Some("Direct push is only available for local threads.".to_string());
        }
        None
    }

    pub(super) fn ai_open_pr_blocker(&self) -> Option<String> {
        self.ai_current_thread_git_action_context("opening PR").err()
    }

    pub(super) fn ai_commit_and_push_for_current_thread(&mut self, cx: &mut Context<Self>) {
        if let Some(reason) = self.ai_publish_blocker().filter(|reason| !reason.is_empty()) {
            let message = format!("Publish unavailable: {reason}");
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, None, cx);
            cx.notify();
            return;
        }

        let context = match self.ai_current_thread_git_action_context("publishing") {
            Ok(context) => context,
            Err(reason) => {
                let message = format!("Publish unavailable: {reason}");
                self.git_status_message = Some(message.clone());
                Self::push_warning_notification(message, None, cx);
                cx.notify();
                return;
            }
        };
        let fallback_commit_message = ai_commit_message_for_thread(
            &self.ai_state_snapshot,
            context.thread_id.as_str(),
            context.branch_name.as_str(),
        );
        let prompt_seed =
            ai_first_prompt_seed_for_thread(&self.ai_state_snapshot, context.thread_id.as_str());
        let latest_agent_message =
            ai_latest_agent_message_for_thread(&self.ai_state_snapshot, context.thread_id.as_str());
        let codex_executable = Self::resolve_codex_executable_path();
        let branch_name = context.branch_name.clone();
        let repo_root = context.repo_root.clone();
        let epoch = self.begin_git_action("Commit and Push", cx);
        let started_at = Instant::now();

        self.git_action_task = cx.spawn(async move |this, cx| {
            let (execution_elapsed, result) = cx
                .background_executor()
                .spawn(async move {
                    let execution_started_at = Instant::now();
                    let result = (|| -> anyhow::Result<(Option<String>, String)> {
                        let commit_message = resolve_ai_commit_message_for_working_copy(
                            AiCodexGenerationConfig {
                                codex_executable: codex_executable.as_path(),
                                repo_root: repo_root.as_path(),
                            },
                            repo_root.as_path(),
                            branch_name.as_str(),
                            prompt_seed.as_deref(),
                            latest_agent_message.as_deref(),
                            &fallback_commit_message,
                        );
                        let commit_message_text = commit_message.as_git_message();
                        let committed_subject = match commit_staged_with_details(
                            repo_root.as_path(),
                            commit_message_text.as_str(),
                        ) {
                            Ok(created) => Some(created.subject),
                            Err(err) if err.to_string().contains("no changes to commit") => None,
                            Err(err) => return Err(err),
                        };

                        let push_result = match push_current_branch(
                            repo_root.as_path(),
                            branch_name.as_str(),
                            true,
                        ) {
                            Ok(()) => Ok(()),
                            Err(err)
                                if err
                                    .to_string()
                                    .contains("publish this branch before pushing") =>
                            {
                                push_current_branch(repo_root.as_path(), branch_name.as_str(), false)
                            }
                            Err(err) if err.to_string().contains("already published") => {
                                push_current_branch(repo_root.as_path(), branch_name.as_str(), true)
                            }
                            Err(err) => Err(err),
                        };
                        push_result?;

                        Ok((committed_subject, branch_name))
                    })();

                    (execution_started_at.elapsed(), result)
                })
                .await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.git_action_epoch {
                        return;
                    }

                    let total_elapsed = started_at.elapsed();
                    this.finish_git_action();
                    match result {
                        Ok((committed_subject, branch_name)) => {
                            debug!(
                                "git action complete: epoch={} action=Commit and Push exec_elapsed_ms={} total_elapsed_ms={} branch={}",
                                epoch,
                                execution_elapsed.as_millis(),
                                total_elapsed.as_millis(),
                                branch_name
                            );
                            let committed = committed_subject.is_some();
                            if let Some(subject) = committed_subject {
                                this.last_commit_subject = Some(subject);
                            }
                            this.request_snapshot_refresh_workflow_only(true, cx);
                            this.request_recent_commits_refresh(true, cx);
                            let message = if committed {
                                format!("Committed and pushed {}", branch_name)
                            } else {
                                format!("Pushed {}", branch_name)
                            };
                            this.git_status_message = Some(message.clone());
                            Self::push_success_notification(message, cx);
                        }
                        Err(err) => {
                            error!(
                                "git action failed: epoch={} action=Commit and Push exec_elapsed_ms={} total_elapsed_ms={} err={err:#}",
                                epoch,
                                execution_elapsed.as_millis(),
                                total_elapsed.as_millis()
                            );
                            let summary = err.to_string();
                            this.git_status_message = Some(format!("Git error: {err:#}"));
                            Self::push_error_notification(
                                format!("Commit and Push failed: {summary}"),
                                cx,
                            );
                        }
                    }

                    cx.notify();
                });
            }
        });
    }

    pub(super) fn ai_open_pr_for_current_thread(&mut self, cx: &mut Context<Self>) {
        if let Some(reason) = self.ai_open_pr_blocker().filter(|reason| !reason.is_empty()) {
            let message = format!("Open PR unavailable: {reason}");
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, None, cx);
            cx.notify();
            return;
        }

        let context = match self.ai_current_thread_git_action_context("opening PR") {
            Ok(context) => context,
            Err(reason) => {
                let message = format!("Open PR unavailable: {reason}");
                self.git_status_message = Some(message.clone());
                Self::push_warning_notification(message, None, cx);
                cx.notify();
                return;
            }
        };
        let fallback_commit_message = ai_commit_message_for_thread(
            &self.ai_state_snapshot,
            context.thread_id.as_str(),
            context.branch_name.as_str(),
        );
        let fallback_review_title = fallback_commit_message.subject.clone();
        let prompt_seed =
            ai_first_prompt_seed_for_thread(&self.ai_state_snapshot, context.thread_id.as_str());
        let latest_agent_message =
            ai_latest_agent_message_for_thread(&self.ai_state_snapshot, context.thread_id.as_str());
        let codex_executable = Self::resolve_codex_executable_path();
        let provider_mappings = self.config.review_provider_mappings.clone();
        let fallback_review_branch_name = ai_branch_name_for_thread(
            &self.ai_state_snapshot,
            context.thread_id.as_str(),
            context.branch_name.as_str(),
            false,
        );
        let review_branch_generation_seed = ai_branch_generation_seed_for_thread(
            &self.ai_state_snapshot,
            context.thread_id.as_str(),
            context.branch_name.as_str(),
        );
        let repo_root = context.repo_root.clone();
        let branch_name = context.branch_name.clone();
        let start_mode = context.start_mode;
        let epoch = self.begin_git_action("Open PR", cx);
        let started_at = Instant::now();

        self.git_action_task = cx.spawn(async move |this, cx| {
            let (execution_elapsed, result) = cx
                .background_executor()
                .spawn(async move {
                    let execution_started_at = Instant::now();
                    let result = (|| -> anyhow::Result<(Option<String>, String, String)> {
                        let review_branch_name = if start_mode == AiNewThreadStartMode::Local {
                            let requested_branch_name = try_ai_branch_name_for_prompt(
                                codex_executable.as_path(),
                                repo_root.as_path(),
                                review_branch_generation_seed.as_str(),
                                &[],
                                false,
                            )
                            .unwrap_or_else(|| fallback_review_branch_name.clone());
                            activate_new_ai_review_branch(
                                repo_root.as_path(),
                                requested_branch_name.as_str(),
                            )?
                        } else {
                            branch_name.clone()
                        };

                        let commit_message = resolve_ai_commit_message_for_working_copy(
                            AiCodexGenerationConfig {
                                codex_executable: codex_executable.as_path(),
                                repo_root: repo_root.as_path(),
                            },
                            repo_root.as_path(),
                            review_branch_name.as_str(),
                            prompt_seed.as_deref(),
                            latest_agent_message.as_deref(),
                            &fallback_commit_message,
                        );
                        let commit_message_text = commit_message.as_git_message();
                        let committed_subject = match commit_staged_with_details(
                            repo_root.as_path(),
                            commit_message_text.as_str(),
                        ) {
                            Ok(created) => Some(created.subject),
                            Err(err) if err.to_string().contains("no changes to commit") => None,
                            Err(err) => return Err(err),
                        };

                        let push_result = match push_current_branch(
                            repo_root.as_path(),
                            review_branch_name.as_str(),
                            true,
                        ) {
                            Ok(()) => Ok(()),
                            Err(err)
                                if err
                                    .to_string()
                                    .contains("publish this branch before pushing") =>
                            {
                                push_current_branch(
                                    repo_root.as_path(),
                                    review_branch_name.as_str(),
                                    false,
                                )
                            }
                            Err(err) if err.to_string().contains("already published") => {
                                push_current_branch(
                                    repo_root.as_path(),
                                    review_branch_name.as_str(),
                                    true,
                                )
                            }
                            Err(err) => Err(err),
                        };
                        push_result?;

                        let review_url = review_url_for_branch_with_provider_map(
                            repo_root.as_path(),
                            review_branch_name.as_str(),
                            &provider_mappings,
                        )?
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "no review URL found for {review_branch_name}; configure review_provider_mappings for self-hosted remotes"
                            )
                        })?;
                        let review_title = committed_subject
                            .clone()
                            .unwrap_or_else(|| fallback_review_title.clone());
                        let review_url = with_review_title_prefill(review_url, review_title.as_str());

                        Ok((committed_subject, review_url, review_branch_name))
                    })();

                    (execution_started_at.elapsed(), result)
                })
                .await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.git_action_epoch {
                        return;
                    }

                    let total_elapsed = started_at.elapsed();
                    this.finish_git_action();
                    match result {
                        Ok((committed_subject, review_url, branch_name)) => {
                            debug!(
                                "git action complete: epoch={} action=Open PR exec_elapsed_ms={} total_elapsed_ms={} branch={} mode={:?}",
                                epoch,
                                execution_elapsed.as_millis(),
                                total_elapsed.as_millis(),
                                branch_name,
                                start_mode
                            );
                            if let Some(subject) = committed_subject {
                                this.last_commit_subject = Some(subject);
                            }
                            this.request_snapshot_refresh_workflow_only(true, cx);
                            this.request_recent_commits_refresh(true, cx);
                            match open_url_in_browser(review_url.as_str()) {
                                Ok(()) => {
                                    let message = format!("Opened PR/MR in browser for {}", branch_name);
                                    this.git_status_message = Some(message.clone());
                                    Self::push_success_notification(message, cx);
                                }
                                Err(err) => {
                                    error!("Open review URL failed: {err:#}");
                                    let summary = err.to_string();
                                    this.git_status_message = Some(format!("Open URL failed: {summary}"));
                                    Self::push_error_notification(
                                        format!("Open review URL failed: {summary}"),
                                        cx,
                                    );
                                }
                            }
                        }
                        Err(err) => {
                            error!(
                                "git action failed: epoch={} action=Open PR exec_elapsed_ms={} total_elapsed_ms={} mode={:?} err={err:#}",
                                epoch,
                                execution_elapsed.as_millis(),
                                total_elapsed.as_millis(),
                                start_mode
                            );
                            let summary = err.to_string();
                            this.git_status_message = Some(format!("Git error: {err:#}"));
                            Self::push_error_notification(format!("Open PR failed: {summary}"), cx);
                        }
                    }

                    cx.notify();
                });
            }
        });
    }
}

fn resolve_ai_commit_message_for_working_copy(
    generation_config: AiCodexGenerationConfig<'_>,
    repo_root: &std::path::Path,
    branch_name: &str,
    prompt_seed: Option<&str>,
    latest_agent_message: Option<&str>,
    fallback_commit_message: &AiCommitMessage,
) -> AiCommitMessage {
    let working_copy_context =
        working_copy_context_for_ai(repo_root, 200, 40_000).ok().flatten();
    let Some(working_copy_context) = working_copy_context else {
        return fallback_commit_message.clone();
    };

    try_ai_commit_message(
        generation_config,
        AiCommitGenerationContext {
            branch_name,
            prompt_seed,
            latest_agent_message,
            changed_files_summary: working_copy_context.changed_files_summary.as_str(),
            diff_patch: working_copy_context.diff_patch.as_str(),
        },
    )
    .unwrap_or_else(|| fallback_commit_message.clone())
}

fn activate_new_ai_review_branch(
    repo_root: &std::path::Path,
    requested_branch_name: &str,
) -> anyhow::Result<String> {
    let mut attempt = 0usize;
    loop {
        attempt = attempt.saturating_add(1);
        let candidate_branch_name = if attempt == 1 {
            requested_branch_name.to_string()
        } else {
            format!("{requested_branch_name}-{attempt}")
        };
        match checkout_or_create_branch_with_change_transfer(
            repo_root,
            candidate_branch_name.as_str(),
            true,
        ) {
            Ok(()) => return Ok(candidate_branch_name),
            Err(err) => {
                if err.to_string().contains("already exists") && attempt < 20 {
                    continue;
                }
                return Err(err);
            }
        }
    }
}
