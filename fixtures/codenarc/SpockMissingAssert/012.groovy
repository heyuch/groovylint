
class MySpec extends spock.lang.Specification {
    def "complexBooleanExpression_MultipleViolations"() {
        expect:
        for (a in [1,2,3]) {
            (boolean) obj.method()
            (Boolean) obj.method()
            obj.method() as boolean
            obj.method() as Boolean
            obj.method() instanceof Boolean
        }
    }
}
        