
class MySpec extends spock.lang.Specification {
    def "xmlMarkupBuilder_NoViolation"() {
        expect:
        "$str" {
            method()
        }
    }
}
        