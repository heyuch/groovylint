
class MyClass {
    void processResults() {
        while(!terminate) {
            def name = rule.name
            allNames.addAll(name)
        }

        while(!terminate) { println 'ok' }
    }
}
        