
class MySpec extends spock.lang.Specification {
    def "assertWithCollectionMethod_NoViolation"() {
        expect:
        if (a == b) {
            assert [1, 2, 3].every { true }
        }
    }
}
        