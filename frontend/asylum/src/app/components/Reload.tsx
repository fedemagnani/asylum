import { fetchData } from "../utils";


const ReloadButton: React.FC<{ state_changer: React.Dispatch<React.SetStateAction<any[]>> }> = ({ state_changer }) => {
    const handleClick = async (event: React.MouseEvent<HTMLButtonElement>) => {
        await fetchData("transactions", state_changer);
    };

    return (
        <button onClick={handleClick} style={buttonStyle}>
            <i className="fas fa-sync-alt">
                
            </i>
        </button>
    );
};

export { ReloadButton };

const buttonStyle: React.CSSProperties = {
    backgroundColor: '#00FF00', // Electric green
    color: '#000000', // Black
    border: 'none',
    padding: '15px 32px',
    textAlign: 'center',
    textDecoration: 'none',
    display: 'inline-block',
    fontSize: '16px',
    margin: '4px 2px',
    cursor: 'pointer',
    fontFamily: 'Courier New, monospace', // Pixel-like font
};

