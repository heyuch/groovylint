
            class MyController {
                def list = {
                    [books: Book.list() ]
                }
                protected boolean isReady() { true }
                def show = {
                    [ book : Book.get(params.id) ]
                }
                private int calculate() { 23 }
            }
        