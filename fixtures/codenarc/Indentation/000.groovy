
class MyClass {
    private static final NAME = "Joe"
    protected int count
    String id = "12345"

    static void reset() { violationCounts = [1:0, 2:0] }

    MyClass(String id) {
        this.id = id
    }

    def myMethod1() { }
    private String doStuff() {
        def internalCounts = [1, 4, 2]
        id.trim()
        new Object().toString()
    }
    static void printReport(String filename) { }
}
        