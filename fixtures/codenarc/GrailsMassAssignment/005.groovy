
            class Person {
                String name
                Boolean isAdmin
            }
            def params = [name: 'John', isAdmin: true]
            def person = new Person(params)
            def person2 = new Person()
            person2.properties = params
        