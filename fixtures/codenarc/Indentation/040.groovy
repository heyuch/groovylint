
class MyFeatureSpec extends Specification {
    MyFeature feature
    Collaborator collaborator = Mock()

    def setup() {
        feature = new MyFeature(collaborator)
    }

    void 'should do something'() {
        given: 'an input'
            Input input = new Input(arg1, arg2)

        when: 'something is done'
            def result = feature.doSomething(input)

        then: 'some interactions occur'
            1 * collaborator.performAction({
                verifyAll(it, Person) {
                    firstname == "Jane"
                    lastname == "Doe"
                    age == 42
                }
            })
            1 * collaborator.performAnotherAction(_) >> { args -> args[0].size() > 3 ? "ok" : "fail" }
            0 * _

        and: 'result is the expected'
            if (true) {
                assert result == expected
            }

        where:
            arg1 | arg2  || expected
            101  | true  || foo
            102  | false || bar
    }

    void 'should do something else'() {
        when:
            def result = feature.doSomethingElse()
        then:
            0 * _
        and:
            if (true) {
                result == "abc123"
            }
    }

    def cleanup() {
        doSomeCleanup()
    }
}
        