
            import groovy.transform.*

            class Person {
                String name
                List addresses

                @Synchronized List getAddresses() {
                    addresses
                }
            }