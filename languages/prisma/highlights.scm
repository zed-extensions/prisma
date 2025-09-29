[
 "datasource"
 "enum"
 "generator"
 "model"
 "view"
 "type"
] @keyword

(comment) @comment
(developer_comment) @comment

(number) @number
(string) @string
(false) @boolean
(true) @boolean
(arguments) @property
(maybe) @punctuation
(call_expression (identifier) @function)
(enumeral) @constant
(identifier) @variable
(column_type (identifier) @type)
(column_type (call_expression (identifier) @type))
(type_declaration_type) @type
(type_declaration (identifier) @type.definition)
(column_declaration (identifier) (column_type (identifier) @type))
(attribute (identifier) @label)
(attribute (call_expression (identifier) @label))
(attribute (call_expression (member_expression (identifier) @label)))
(block_attribute_declaration (identifier) @label)
(block_attribute_declaration (call_expression (identifier) @label))
(type_expression (identifier) @property)

"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"=" @operator
"@" @operator
"@@" @operator
