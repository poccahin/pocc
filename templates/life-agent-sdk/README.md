# Life++ Agent SDK Template

A minimal TypeScript SDK template that you can publish to npm quickly.

## Project structure

```
life-agent-sdk
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   └── agent/
│       ├── core.ts
│       ├── memory.ts
│       └── tool.ts
└── .github/workflows/publish.yml
```

## Local development

```bash
npm install
npm run build
```

## Publish checklist

1. Login:
   ```bash
   npm login
   ```
2. Update package name/version in `package.json`.
3. Preview package contents:
   ```bash
   npm pack
   ```
4. Publish:
   ```bash
   npm publish --access public
   ```

## Automated release via GitHub Actions

Add an `NPM_TOKEN` repository secret, then push a semver tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```
