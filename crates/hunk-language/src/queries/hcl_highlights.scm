(comment) @comment

(bool_lit) @boolean
(numeric_lit) @number

(string_lit) @string
(template_literal) @string
(heredoc_identifier) @string.special

(identifier) @variable

(attribute
  (identifier) @property)

(block
  (identifier) @keyword)

(block
  (string_lit) @string.special)

[
  "="
] @operator

[
  "{"
  "}"
  "["
  "]"
  "("
  ")"
] @punctuation.bracket

[
  "."
  ","
] @punctuation.delimiter

[
  (template_directive_start)
  (template_directive_end)
] @keyword

[
  (template_interpolation_start)
  (template_interpolation_end)
] @punctuation.special
