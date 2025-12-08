# Zed Prisma

A [Prisma](https://www.prisma.io/) extension for [Zed](https://zed.dev).

## Development

To develop this extension, see the [Developing Extensions](https://zed.dev/docs/extensions/developing-extensions) section of the Zed docs.

## Pin language server to a specific Prisma version

You can pin the Prisma language server version through your Zed settings. To use the extension's default Prisma v6 language server, set `pinToPrisma6` to `true`:

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

To pin to a custom Prisma language server release, provide `pinnedPrismaVersion`. Supplying this value also enables pinning, even if `pinToPrisma6` is omitted:

```json
"lsp": {
    "prisma-language-server": {
        "settings": {
            "prisma": {
                "pinnedPrismaVersion": "6.5.0"
            }
        }
    }
}
```
