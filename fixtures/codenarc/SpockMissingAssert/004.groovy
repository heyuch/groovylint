
class MySpec extends spock.lang.Specification {
    def "nestedClosureInFor_NoViolation"() {
        given:
        for (a in [1,2,3]) {
            then:
            methodCall {
                !myCondition()
            }
        }
    }
}
        