
public class MySpec extends spock.lang.Specification {

    def 'test1'() {
        when:
        if (bar()) {
            assert true
        }
        bar()

        then:
        [1, 2, 3].each {
            assert true
        }
    }

    def 'test2'() {
        expect:
        assert true
    }

    private static boolean bar() {
        def abc = baz.method()
        if (abc) {
            false
        } else {
            true
        }
    }
}
        