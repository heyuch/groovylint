
            class Person {
                String name
                Boolean isAdmin
            }
            def params = [name: 'John', isAdmin: true]
            def person = Person.get(1)
            person.properties = params
        