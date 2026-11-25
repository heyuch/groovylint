
    public class MySpec extends spock.lang.Specification {
        def "defCollectionMethod_NoViolation"() {
            given:
            def foo = method1()
            method2()

            when:
            method3()

            then:
            def bar = method4()

            and:
            def baz = [1,2,3].find {
                try {
                    method5()
                    true
                } catch (AssertionError ignored) {
                    false
                }
            }
        }
}
        