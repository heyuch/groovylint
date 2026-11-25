
class MySpec extends spock.lang.Specification {
    def "nonTopLevelLabels_MultipleViolations"() {
        given:
        123
        expect:
        !myCondition()
        for (a in [1,2,3]) {
            then:
            [1,2,3].each {
                !myCondition()
            }
            [1,2,3].eachWithIndex { it, i ->
                !myCondition()
            }
            3.times {
                !myCondition()
            }
        }
    }
}
        