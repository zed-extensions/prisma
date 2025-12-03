# Zed Prisma

A [Prisma](https://www.prisma.io/) extension for [Zed](https://zed.dev).

## Development

To develop this extension, see the [Developing Extensions](https://zed.dev/docs/extensions/developing-extensions) section of the Zed docs.

## Pin language server to Prisma v6

To pin the language server to Prisma v6, add the following configuration to your Zed settings:

```json
"lsp": {
    "prisma-language-server": {
        "settings": {
            "prisma": {
                "pinToPrisma6": true
            }
        }
    }
}
```
