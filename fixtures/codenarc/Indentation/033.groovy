
class MyClass {
    void processResults() {
        for (Rule rule: validRules) {
            def name = rule.name
            allNames.addAll(name)
        }

        for(int i=1; i < 99; i++) { println i }
    }
}
        