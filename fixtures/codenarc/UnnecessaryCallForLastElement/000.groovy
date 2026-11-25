
                def x = [0, 1, 2]
                def a = x.last()
                def b = x[-1]
                def c = x.getAt(-1)
                def d = x.get(z.size() -1)     // different objects
                def e = x.get(z.length -1)     // different objects
                def f = x.getAt(z.size() -1)   // different objects
                x.get(x.size() - 2)
                x.get(x.length - 2)
        