
public class SomeFancyClass  { // I'm not a spec
    def "my first feature"() {
        expect: false
    }

    @IgnoreRest
    def "my second feature"() {
        given: def a = 2

        when: a *= 2

        then: a == 4
    }
}
        