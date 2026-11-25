
                import spock.lang.Requires
                import spock.lang.Specification
                
                class SomeSpec extends Specification {
                    @Requires({ true })
                    def 'asd'() {
                        expect:
                        true
                    }
                }
            