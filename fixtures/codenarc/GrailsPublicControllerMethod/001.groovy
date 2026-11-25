
            class MyController {
                boolean isThisNecessary() { false }
                def list = { [books: Book.list() ] }
                void doSomething() {
                    // this does not need to be public
                }
            }
        