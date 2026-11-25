
            doStuff({ -> println 111 } as Runnable,
                 { -> println 222 } as Runnable)

            1 * service.calculateSmth(
                { Bean1 request1 -> request1.type == MAIN } as Bean1,
                { Bean2 request2 -> request2.type == MAIN } as Bean2) == 0.50
        