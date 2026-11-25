
class IndentationTest {
    private static final CACHE2 = build(
       new CacheLoader<String>() { })     // Violation - wrong column

    void someMethod() {
        CacheBuilder2.build(
    new CacheLoader2<String>() {           // Violation - valid indent offset, not indented enough
        void load2(String key) {
            key.upperCase()
        }
                 })
    }

    void doStuff() {
         new BigDeal() {                   // Violation - wrong column
            String errorMessage() { }
        }

      new Runner() { }.run()               // Violation - wrong column

        CacheBuilder2.build(
         new CacheLoader2() { })           // Violation - wrong column

        def processor2 =
          new Processor2() { }             // Violation - Wrong column
    }
}
        