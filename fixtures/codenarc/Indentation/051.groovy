
class IndentationTest {
    static final CACHE = build(
    new Cache<String>() { })          // Known Limitation: valid indent offset, but not indented enough; no violation

    def list = [1, 2,
      new Processor() { }]             // Known Limitation: Skips List expressions; Wrong column, but no violation
}
        