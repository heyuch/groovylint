import spock.lang.*
public class MySpec extends Specification {
    @IgnoreRest
    def "my first feature"() {
        expect: false
    }

    def "my second feature"() {
        given: def a = 2

        when: a *= 2

        then: a == 4
    }
}
        