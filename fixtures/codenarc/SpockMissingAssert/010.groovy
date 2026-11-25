
class MySpec extends spock.lang.Specification {
    def "complexBooleanExpression_MultipleViolations"() {
        expect:
        for (a in [1,2,3]) {
            !obj.method()
            !'foo'
            obj.method() ? obj.isBoolean() : obj.asBoolean()
            obj.method() ?: obj.isBoolean()
        }
    }
}
        