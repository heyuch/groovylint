
            import static cucumber.api.groovy.Hooks.Before

            @groovy.transform.Field String myVar

            Before {
                doStuff()
            }

            private void doStuff() {
                myVar = '50'
            }
            