---
name: caddy-subdirectory-static-sites
description: Serve multiple static sites under subdirectories of one domain using Caddy
---

## What
Serves multiple independent static sites (each with their own `index.html`) under subdirectory paths like `example.com/product1`, `example.com/product2` — all from one Caddy server block.

## Why
A single `root * /apps` with `file_server` does NOT auto-serve `index.html` from subfolders. Caddy needs explicit `handle` blocks per subdirectory.

## How

### Caddyfile Pattern
```
example.com {
    # Main site (root)
    handle {
        root * /path/to/sites/main
        file_server
    }

    # Each product subdirectory
    handle /product1 {
        root * /path/to/sites/product1
        try_files /index.html
        file_server
    }

    handle /product2 {
        root * /path/to/sites/product2
        try_files /index.html
        file_server
    }
}
```

### Key: `try_files /index.html`
Without this, requesting `/product1` (no trailing slash) tries to serve a file called `product1` from the root directory and 404s. `try_files /index.html` rewrites it to serve the index file.

### Directory Structure
```
/path/to/sites/
  main/index.html
  product1/index.html
  product2/index.html
```

## Gotchas
- Each product folder needs its own `index.html` — no index in the folder = no content served
- CSS/JS/image paths inside each `index.html` should be relative or absolute from the subdirectory root
- If products have their own assets (images, CSS), put them in the same subfolder and reference relatively
- Links between products should use absolute paths (`/product1`, `/product2`)
