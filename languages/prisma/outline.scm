; Datasource blocks
(datasource_declaration
    "datasource" @context
    (identifier) @name
    (statement_block)
) @item

; Models
(model_declaration
    "model" @context
    (identifier) @name
    (statement_block)
) @item

; Views
(view_declaration
    "view" @context
    (identifier) @name
    (statement_block)
) @item

; Generator blocks
(generator_declaration
    "generator" @context
    (identifier) @name
    (statement_block)
) @item

; Types
(type_declaration
    "type" @context
    (identifier) @name
    (statement_block)
) @item

; Enums
(enum_declaration
    "enum" @context
    (identifier) @name
    (enum_block)
) @item

; Comments
(comment) @documentation
