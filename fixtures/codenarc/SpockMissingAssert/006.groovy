
class MySpec extends spock.lang.Specification {
    def "nestedClosureInCollectionLoops_SingleViolation"() {
        given:
        123
        expect:
        for (a in [1,2,3]) {
            methodCall({ it in ["a","b","c"] }).each {
                with(it) {
                    !myCondition1()
                }
                assert !myCondition2()
                !myCondition3()
            }
        }
    }
}
        