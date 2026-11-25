public class MySpec extends spock.lang.Specification {
    @spock.lang.IgnoreRest
    def "my first feature"() {
        expect: false
    }

    def "my second feature"() {
        given: def a = 2

        when: a *= 2

        then: a == 4
    }
}
        