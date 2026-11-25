
                class MyClass {
                    def myMethod() {
                        def greeting = fullname ?:
                        'you'
                        def doubleElvis = fullname ?: lastname ?:
                        'me'
                        def newLineElvis = fullname                         ?: 'you'
                    }
                }
            