"param" @keyword
"dynamicparam" @keyword
"begin" @keyword
"process" @keyword
"end" @keyword
"if" @keyword
"elseif" @keyword
"else" @keyword
"switch" @keyword
"foreach" @keyword
"for" @keyword
"while" @keyword
"do" @keyword
"until" @keyword
"function" @keyword
"filter" @keyword
"workflow" @keyword
"break" @keyword
"continue" @keyword
"throw" @keyword
"return" @keyword
"exit" @keyword
"trap" @keyword
"try" @keyword
"catch" @keyword
"finally" @keyword
"data" @keyword
"inlinescript" @keyword
"parallel" @keyword
"sequence" @keyword

"-as" @operator
"-eq" @operator
"-ge" @operator
"-gt" @operator
"-le" @operator
"-like" @operator
"-lt" @operator
"-match" @operator
"-ne" @operator
"-replace" @operator
"-split" @operator
"-and" @operator
"-or" @operator
"-xor" @operator
"+" @operator
"-" @operator
"/" @operator
"\\" @operator
"%" @operator
"*" @operator
".." @operator
"-not" @operator

";" @punctuation.delimiter

(string_literal) @string
(integer_literal) @number
(real_literal) @number

(command
  command_name: (command_name) @function)

(function_statement
  (function_name) @function)

(invokation_expression
  (member_name) @function)

(member_access
  (member_name) @property)

(command_invokation_operator) @operator
(type_spec) @type
