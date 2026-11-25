
            class Person {
                String name
                Boolean isAdmin
            }
            params = [name: 'John', isAdmin: true]
            def person = new Person(params)
        