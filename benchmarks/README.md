# Benchmarks

Two layers are used:

1. **Criterion microbenchmarks** for parsing and rule execution:
   ```bash
   cargo bench -p groovylint --bench linter
   ```
2. **End-to-end CodeNarc comparison** on a real Groovy corpus:
   ```bash
   cargo build --release -p groovylint
   CODENARC_CLASSPATH="/path/to/codenarc-libs/*" \
     python3 benchmarks/compare_codenarc.py \
     --corpus /path/to/groovy-project \
     --iterations 5
   ```

The comparison script first runs `groovylint` to find files it can parse, stages
only those files in a temporary corpus, then runs both tools on the same staged
files with CodeNarc's `rulesets/basic.xml`. This avoids comparing a full parse
against a partial parse. It also prints normalized finding differences before
the timing results, because performance numbers are only meaningful when both
tools are doing comparable work.

## CodeNarc jars

A reproducible way to obtain CodeNarc 4.0.0 and its runtime dependencies is a
throwaway Maven project:

```xml
<project xmlns="http://maven.apache.org/POM/4.0.0">
  <modelVersion>4.0.0</modelVersion>
  <groupId>local</groupId>
  <artifactId>codenarc-run</artifactId>
  <version>1.0</version>
  <dependencies>
    <dependency>
      <groupId>org.codenarc</groupId>
      <artifactId>CodeNarc</artifactId>
      <version>4.0.0</version>
    </dependency>
    <dependency>
      <groupId>ch.qos.logback</groupId>
      <artifactId>logback-classic</artifactId>
      <version>1.5.18</version>
    </dependency>
  </dependencies>
</project>
```

```bash
mvn -q dependency:copy-dependencies -DoutputDirectory=lib
export CODENARC_CLASSPATH="$PWD/lib/*"
```

## Suggested public corpora

- [CodeNarc](https://github.com/CodeNarc/CodeNarc) itself
  (`src/main/groovy`): many real Groovy rule implementations and test utilities.
- [Apache Groovy](https://github.com/apache/groovy)
  (`src/main/groovy` plus relevant `subprojects/*/src/main/groovy`): a large,
  diverse compiler/runtime codebase.
- [Spock](https://github.com/spockframework/spock) and
  [Grails](https://github.com/grails/grails-core) for test/spec and framework
  Groovy styles.

Pin the commit when recording results. Do not commit downloaded corpora.

## Reporting

Record at least:

- CPU/OS/JVM versions;
- corpus URL and commit;
- number of files and parseable files;
- CodeNarc and groovylint versions;
- thread count;
- median/min/max wall time and files/second;
- finding parity (or known differences).

Current local results are recorded in [`RESULTS.md`](RESULTS.md). The first run found 0 normalized differences on both public corpora.
