# Build/Run instructions

(these build instructions assume that a working rust-stable toolchain is installed)

```bash
git clone https://github.com/Ex-32/cs128h-project.git rs-shell
cd rs-shell
cargo run
```

this will launch an interactive shell instance, where you can run **basic** commands, you can also try out the `-c` flag to run a single command:

```bash
cargo run -- -c '<COMMAND>'
```
