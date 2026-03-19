;; Based on the tree-sitter-markdown block highlight query, which is under the MIT license.
;; Source:
;; https://github.com/tree-sitter-grammars/tree-sitter-markdown/blob/master/tree-sitter-markdown/queries/highlights.scm

(atx_heading
  (inline) @markup.heading)

(setext_heading
  (paragraph) @markup.heading)

[
  (atx_h1_marker)
  (atx_h2_marker)
  (atx_h3_marker)
  (atx_h4_marker)
  (atx_h5_marker)
  (atx_h6_marker)
  (setext_h1_underline)
  (setext_h2_underline)
] @punctuation.special

[
  (link_title)
  (indented_code_block)
  (fenced_code_block)
] @markup.raw.block

(fenced_code_block_delimiter) @punctuation.delimiter

(code_fence_content) @embedded

(link_destination) @markup.link.url

(link_label) @markup.link

[
  (list_marker_plus)
  (list_marker_minus)
  (list_marker_star)
  (list_marker_dot)
  (list_marker_parenthesis)
  (thematic_break)
  (block_continuation)
  (block_quote_marker)
] @punctuation.special

(backslash_escape) @string.escape
