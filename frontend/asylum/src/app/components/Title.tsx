// Here we define the title component
import './Title.css'
const Title = () => {
    return (
        <pre>
            {
                `
_______  _______           _                 _______
(  ___  )(  ____ \\|\\     /|( \\      |\\     /|(       )
| (   ) || (    \\/( \\   / )| (      | )   ( || () () |
| (___) || (_____  \\ (_) / | |      | |   | || || || |
|  ___  |(_____  )  \\   /  | |      | |   | || |(_)| |
| (   ) |      ) |   ) (   | |      | |   | || |   | |
| )   ( |/\\____) |   | |   | (____/\\| (___) || )   ( |
|/     \\|\\_______)   \\_/   (_______/(_______)|/     \\|
    `
            }
        </pre>
    );
};
export {Title};