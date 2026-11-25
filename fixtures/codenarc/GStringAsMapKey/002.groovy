
            def x = 'something'
            Map map = ["outer $x" :
                          ["nested $x" : 'invalid']
             ]
        