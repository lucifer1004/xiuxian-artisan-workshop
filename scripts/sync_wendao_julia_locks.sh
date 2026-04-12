#!/usr/bin/env bash
set -euo pipefail

root="${PRJ_ROOT:?PRJ_ROOT must be set}"
data_home="${PRJ_DATA_HOME:?PRJ_DATA_HOME must be set}"

arrow_rev="$(git -C "$data_home/arrow-julia" rev-parse HEAD)"
grpcserver_rev="$(git -C "$data_home/gRPCServer.jl" rev-parse HEAD)"
wendaoarrow_rev="$(git -C "$data_home/WendaoArrow.jl" rev-parse HEAD)"
wendaocodeparser_rev="$(git -C "$data_home/WendaoCodeParser.jl" rev-parse HEAD)"
omparser_rev="$(git -C "$data_home/OMParser.jl" rev-parse HEAD)"
omparser_tree="$(git -C "$data_home/OMParser.jl" rev-parse HEAD^{tree})"

python3 "$root/scripts/sync_wendao_julia_locks.py" \
  "$data_home" \
  "$arrow_rev" \
  "$grpcserver_rev" \
  "$wendaoarrow_rev" \
  "$wendaocodeparser_rev" \
  "$omparser_rev" \
  "$omparser_tree"

direnv exec "$root" bash -lc 'cd "$PRJ_DATA_HOME/WendaoArrow.jl" && julia --project=. -e "using Pkg; Pkg.resolve(); Pkg.instantiate()"'
direnv exec "$root" bash -lc 'cd "$PRJ_DATA_HOME/WendaoCodeParser.jl" && julia --project=. -e "using Pkg; Pkg.resolve(); Pkg.instantiate()"'
direnv exec "$root" bash -lc 'cd "$PRJ_DATA_HOME/WendaoSearch.jl" && julia --project=. -e "using Pkg; Pkg.resolve(); Pkg.instantiate()"'
