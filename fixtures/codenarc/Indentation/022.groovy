
class MyClass {
    void test_processResults() {
        shouldFailWithMessageContaining('results') { processor.processResults(null) }
    }
}
        