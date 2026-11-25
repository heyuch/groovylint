
            import a.b.SomeClass
            import d.e.OtherClass as OC
            import f.g.ThirdClass as TC
            class ABC extends a.b.SomeClass {
                def name = d.e.OtherClass.name
                def info = f.g.ThirdClass.name + ":" + TC.metaClass.name
            }
        