
            import org.gcontracts.annotations.Requires
            class CodenarcViolationService {
                @Requires({arg})
                def violate(arg) {
                    println 'foo bar'
                }
            }
            