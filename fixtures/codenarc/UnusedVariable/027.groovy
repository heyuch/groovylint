
            import cucumber.api.groovy.EN
            import groovy.transform.Field

            import static cucumber.api.groovy.Hooks.Before

            metaClass.mixin(EN)

            @Field String myVar

            Before {
                doStuff()
            }

            private void doStuff() {
                myVar = '50'
            }
            