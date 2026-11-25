
            def myService = [
            getCount:{ id, name ->

                assert id == 99
                assert name == 'Joe'
                return 1
            },
            doStuff: {

            }
        ] as MyService
        