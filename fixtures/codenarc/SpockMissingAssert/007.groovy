
class MySpec extends spock.lang.Specification {
    def "complexBooleanExpression_MultipleViolations"() {
        expect:
        for (a in [1,2,3]) {
            myCondition() == myVariable
            myCondition() != myVariable
            obj.method().myCondition() == obj.method().myVariable
            obj.method().myCondition() != obj.method().myVariable
            obj.method() == obj.method()
            obj.method() != obj.method()
            obj.method() < obj.method()
            obj.method() <= obj.method()
            obj.method() > obj.method()
            obj.method() >= obj.method()
            obj.method() === obj.method()
            obj.method() !== obj.method()
        }
    }
}
        