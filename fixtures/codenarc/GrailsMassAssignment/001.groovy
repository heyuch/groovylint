
            class Person {
                String name
                Boolean isAdmin
            }
            def params = [name: 'John', isAdmin: true]
            def person = new Person()
            def staticData = [name: 'John']
            person.properties = staticData
            bindData(person,params,include=['name'])
            bindData(person,params,exclude=['isAdmin'])
        