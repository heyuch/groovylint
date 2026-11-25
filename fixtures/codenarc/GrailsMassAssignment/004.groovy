
            class Person {
                String name
                Boolean isAdmin
            }
            a = {
                def params = [name: 'John', isAdmin: true]
                def person = Person.get(1)
                person.properties = params
            }
            a()
        