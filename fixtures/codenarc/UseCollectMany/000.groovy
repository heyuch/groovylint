
            def l = [1, 2, 3, 4]
            l.collect{ [it, it*2] }
            l.flatten()

            def res = l.collect{ [it, it*2] }
            res.flatten()
        