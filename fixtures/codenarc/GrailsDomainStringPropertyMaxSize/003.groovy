
            class Person {
                String firstName
                String lastName

                static constraints = [size: 1..100]
                static mapping = [firstName: 'text']
            }
        