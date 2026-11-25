
            class Person {
                String name
                List addresses

                @groovy.transform.Synchronized List getAddresses() {
                    addresses
                }
            }