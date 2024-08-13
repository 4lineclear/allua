test:
    cargo nextest run

md:
    mdflc ./notes/

todo:
    rg "todo|FIX|TODO|HACK|WARN|PERF|NOTE|TEST" ./src/


cov:
    cargo llvm-cov --html
