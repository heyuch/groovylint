
class MyFeatureSpec extends Specification {
    void 'should do something'() {
        when:
            def result = new MyFeature().doSomething()

        then:
        if (true) {
            result == "abc123"
        }
    }

    void 'should be true'() {
        expect:
            true
    }
}
        