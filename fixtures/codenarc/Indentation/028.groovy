
class MyClass {
    void doStuff(String name) {
        doWith('results') {
            println  name
            processResults(name)
        }
        doWith(
            name,
            null) {
            println  name
            processResults(name)
        }
    }
}
        