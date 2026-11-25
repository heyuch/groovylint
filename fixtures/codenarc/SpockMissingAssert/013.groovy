
class MySpec extends spock.lang.Specification {
    def "complexBooleanExpression_MultipleViolations"() {
        expect:
        for (a in [1,2,3]) {
            myCondition()
            isMyCondition()
            hasMyCondition()
            !myCondition()
            obj.method().myCondition()
            obj.method().isMyCondition()
            obj.method().hasMyCondition()
            !obj.method().myCondition()
            obj.method() in [1,2,3]
        }
    }
}
        