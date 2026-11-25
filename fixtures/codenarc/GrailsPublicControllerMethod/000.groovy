
            class MyController {
                int someField
                def list = {
                    [books: Book.list() ]
                }
                void doSomething() {
                    // this does not need to be public
                }
            }
        