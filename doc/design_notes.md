# Neptune Block Explorer Design notes

## Initial functionality

* show tip info (digest, height, etc)
* lookup block by selectors: height, digest, genesis, tip
* display block info
* something about generation addresses.  (ask Alan, todo)
* lookup UTXO, show #of confirmations. (todo)  confirm utxo is confirmed

## Block Explorer RPC calls
* tip_info
* block_info
* utxo_info

## Neptune RPC calls to support Block Explorer
* tip_info
* block_info
* utxo_info


## Architecture

Block Explorer is comprised of:
* Server:
    * RPC Server (backend)
    * GUI Server (for serving html/js/wasm)
* Client:
    * Web Client (browser, mobile, etc)
    * Rpc Client (browser, any 3rd party app)

RPC Server and GUI Server are logical components of the same
server instance.  The server is built with axum framework.

Server goals:
* provide basic "block explorer" functionality
* fast response times
* public access: no authentication necessary, read-only.
* simple maintainable code
    * RPC Specific:
        * simple self-documenting public APIs
        * keep response data small.  avoid huge responses
    * GUI Specific:
        * simple, hand-crafted, maintainable HTML
        * javascript-free

GUI Server is built with:
* `axum` for server/routing
* [boilerplate](https://crates.io/crates/boilerplate) for templates with embedded rust
* [pico-css](https://picocss.com/) for responsive light/dark themes, js free.

For simplicity and efficiency, the GUI Server calls neptune-core APIs directly
rather than calling the neptune-explorer RPC APIs over http or internally.

## Client/Server communication

Server and Client communicate via http/RPC or http/html.  The initial RPC
mechanism is REST using axum's built-in rest support.  REST has the benefit that
it can be accessed via a web-browser. We may add JSON-RPC support later.

## Future Clients

It is envisioned that more advanced clients may be created in the future.  For
example something like `Dioxus` or `Leptos` could be used to create client(s) for WASM,
mobile, desktop, and perhaps even tui/cli.  Such client(s) should live in their
own repo and communicate with the block-explorer server only via RPC.