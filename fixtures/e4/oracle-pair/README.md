# E4-01 / E4-02 / E4-03 oracle dual pairs (TEST-008 slices)

**Status:** **done (MVP)** — default-calls1 (E4-01) + blocks-calls1 (E4-02) + calls2-default (E4-03) beyond `fixtures/e4/dual-sink/`  
**Not:** full TEST-008 / TEST-003 corpus; product `format=dual`; CLI v6 collection default; opcode/`entersub`; live attach blocks-780; live attach SUB_ENTRY **27**

| File | Producer |
|------|----------|
| `default_calls1_v5.nytprof` | Pinned oracle `fixtures/v5/default-calls1/nytprof.out` (isolated `PERL5LIB`, never `crates/`) |
| `default_calls1_v6.nytprof` | Product D1-A `perl -d:NYTProf` `format=v6` on the same `workload.pl` |
| `blocks_calls1_v5.nytprof` | Pinned oracle `fixtures/v5/blocks-calls1/nytprof.out` (isolated `PERL5LIB`, never `crates/`) |
| `blocks_calls1_v6.nytprof` | Product D1-A `perl -d:NYTProf` `format=v6` on the same `workload.pl` |
| `calls2_default_v5.nytprof` | Pinned oracle `fixtures/v5/calls2-default/nytprof.out` (isolated `PERL5LIB`, never `crates/`) |
| `calls2_default_v6.nytprof` | Product D1-A `perl -d:NYTProf` `format=v6` on the same `workload.pl` |

Advertised **count** surfaces from shipped `ProfileModel::from_path` / `report --json`: leaf **15** / mid **3** / mid→leaf **15**. DISCOUNT / TIME_LINE / TIME_BLOCK / A4 **780** / SUB_ENTRY **27** / wall times remain TEST-008 residual (product v6 half is `DB::sub`/`DB::DB`, not opcode/`calls=2`). E4/oracle equality must **not** pass `--allow-lossy`.

Regenerate (needs CC + zstd/lz4):

```sh
./scripts/packaging/gen_e4_oracle_pair.sh
```
