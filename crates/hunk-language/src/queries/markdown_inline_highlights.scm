;; Based on gpui-component / tree-sitter-markdown inline highlight queries.
;; Adapted to Hunk's supported syntax color names.

[
  (emphasis_delimiter)
  (code_span_delimiter)
] @punctuation.delimiter

(emphasis) @emphasis

(strong_emphasis) @emphasis.strong

[
  (code_span)
  (link_title)
] @text.literal

[
  (link_destination)
  (uri_autolink)
  (email_autolink)
] @link_uri

[
  (link_label)
  (link_text)
  (image_description)
] @link_text

[
  (backslash_escape)
  (hard_line_break)
] @string.escape

(image
  [
    "!"
    "["
    "]"
    "("
    ")"
  ] @punctuation.delimiter)

(inline_link
  [
    "["
    "]"
    "("
    ")"
  ] @punctuation.delimiter)

(shortcut_link
  [
    "["
    "]"
  ] @punctuation.delimiter)
