
class MySpec extends spock.lang.Specification {
    def "complexBooleanExpression_MultipleViolations"() {
        expect:
        for (a in [1,2,3]) {
            myVar.asBoolean()
            [1,2,3].any { myCondition() }
            [1,2,3].contains(4)
            [1,2,3].every { myCondition() }
            abc.equals(myVariable)
        }
    }
}
        