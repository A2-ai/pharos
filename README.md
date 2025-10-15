```
cargo run --release -- nonmem init
cargo run --release -- nonmem check components/nonmem/models/BQL/bql.mod
cargo run --release -- nonmem run components/nonmem/models/BQL/bql.mod --overwrite
cargo run --release -- nonmem run components/nonmem/models/BQL/bql.mod --overwrite --output-dir="_{{name}}.dir"
cargo run --release -- nonmem run components/nonmem/models/nmexample/nmexample.mod --overwrite
cargo run --release -- nonmem copy --from=components/nonmem/models/BQL/bql.mod --to=components/nonmem/models/BQL/bql2.mod --update=theta,omega --jitter theta:0.2 --jitter omega:0.3 --overwrite  --jitter-excluded=THETA1

cargo run --release -- nonmem lineage components/nonmem/models/BQL/
```
