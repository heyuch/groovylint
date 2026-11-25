
class MyClass {
    def processResults() {
        return list.findAll { it instanceof Date }
    }
    def processOtherResults() {
        return list.
            findAll { it instanceof Date }
    }
}
        