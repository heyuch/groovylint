
            class Person {
                String name
                Boolean isAdmin
            }
            def bindingMap = [name: 'John', isAdmin: true]
            def person = new Person()
            def p2 = new Person("It is currently ${ new Date() }")
            def p3 = new Person(bindingMap)
            person.name = bindingMap['name']
            person.isAdmin = bindingMap.isAdmin
            person.properties = "It is currently ${ new Date() }"
        