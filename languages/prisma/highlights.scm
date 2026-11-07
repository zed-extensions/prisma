; Prisma Tree-sitter Highlighting Queries for Zed
; Designed for rich, semantic syntax highlighting on parity with TypeScript
; Standard Keywords
[
  "datasource"
  "enum"
  "generator"
  "model"
  "type"
  "view"
] @keyword

; Declarations (models, views, enums, mixins, types, generators, datasources)
(model_declaration
  (identifier) @type)

(view_declaration
  (identifier) @type)

(enum_declaration
  (identifier) @type)

(type_declaration
  (identifier) @type)

(datasource_declaration
  (identifier) @type)

(generator_declaration
  (identifier) @type)

; Contextual keywords and modifiers in Prisma 8:
; - mixin / contract: mixin composition and contract spaces
; - namespace: Postgres schemas / namespaces
; - types: top-level type aliases
; - policy_*: RLS policy declarations (policy_select, policy_insert, etc.)
((identifier) @keyword
  (#any-of? @keyword
    "mixin" "contract" "namespace" "types" "policy_select" "policy_insert" "policy_update"
    "policy_delete" "policy_all"))

((ERROR
  (ERROR) @keyword)
  (#match? @keyword
    "^(mixin|contract|namespace|types|policy_select|policy_insert|policy_update|policy_delete|policy_all)$"))

; Comments
(comment) @comment

(developer_comment) @comment

; Literals
(number) @number

(string) @string

(false) @boolean

(true) @boolean

(enumeral) @constant

; Prisma 8 control policies (@@control(managed), @@control(tolerated), etc.)
((identifier) @constant
  (#any-of? @constant "managed" "tolerated" "external" "observed"))

; Field declarations and assignment properties
(column_declaration
  (identifier) @property)

(assignment_expression
  (variable) @property)

; Field types
(column_type
  (identifier) @type)

(column_type
  (call_expression
    (identifier) @type))

(type_declaration_type) @type

; Namespaced types (e.g. temporal.UpdatedAt, sql.Ulid, pgvector.Vector, postgis.Geometry, etc.)
(member_expression
  (identifier) @type
  (property_identifier) @type)

; Functions inside attribute arguments or default expressions (e.g. autoincrement(), uuid(7), ulid(), now(), env())
(call_expression
  (identifier) @function)

; SQL literal tag (e.g. where: sql`...`, Raw(sql`...`))
((identifier) @function
  (#eq? @function "sql"))

; Attribute argument properties (e.g. fields: [userId], references: [id], expression:, where:, name:, unique:)
(type_expression
  (identifier) @property)

; Identifier references inside attribute arrays (e.g. [userId], [id], [search])
(array
  (identifier) @variable.member)

; Attributes (e.g. @id, @unique, @default, @map, @relation, @onCreate, @onUpdate, @@map, @@index, @@check, @@control, @@fullTextIndex, @@rls)
"@" @label

"@@" @label

(attribute
  (identifier) @label)

(attribute
  (call_expression
    (identifier) @label))

(attribute
  (call_expression
    (member_expression
      (identifier) @label
      (property_identifier) @label)))

(block_attribute_declaration
  (identifier) @label)

(block_attribute_declaration
  (call_expression
    (identifier) @label))

(block_attribute_declaration
  (call_expression
    (member_expression
      (identifier) @label
      (property_identifier) @label)))

; Prisma 8 mixin inclusions (+Timestamps) and error recovery
"+" @operator

; SQL template strings in backticks (e.g. sql`deleted_at IS NULL`)
((ERROR
  (ERROR) @string)
  (#match? @string "^`"))

((ERROR
  (ERROR) @label)
  (#match? @label "^[a-z]"))

((ERROR
  (ERROR) @type)
  (#match? @type "^[A-Z]"))

; Punctuation & Operators
(maybe) @punctuation.special

"=" @operator

":" @punctuation.delimiter

"," @punctuation.delimiter

"(" @punctuation.bracket

")" @punctuation.bracket

"[" @punctuation.bracket

"]" @punctuation.bracket

"{" @punctuation.bracket

"}" @punctuation.bracket
