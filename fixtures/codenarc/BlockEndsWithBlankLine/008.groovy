
            def myService = [
                getCount:{ id, name ->
                    assert id == 99
                    assert name == 'Joe'
                    return 1

                },
                doStuff:{

                },
                doOtherStuff:{ id ->

                }] as MyService

            def closure1 = { id ->

            }

            def closure2 = { id ->

            };

            def list = [{ id ->

            },
            123]

            myList.each { item ->

            }
        