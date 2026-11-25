
            import javax.servlet.http.Cookie
            import javax.sql.DataSource
            import com.example.OtherClass

            class MyClass {
                void doStuff(javax.servlet.http.Cookie cookie, Cookie[] cookies) {
                    def dataSource = [:] as javax.sql.DataSource
                    DataSource dataSource2 = wrap(dataSource)
                }
            }
        