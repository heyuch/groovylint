
class MySpec extends spock.lang.Specification {
    def "complexBooleanExpression_MultipleViolations"() {
        expect:
        for (a in [1,2,3]) {
            ~'foo'
            text =~ /match/
            text !=~ /match/
            text ==~ /match/
        }
    }
}
        